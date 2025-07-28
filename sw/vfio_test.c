// Written by AI, cleaned up by a human

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/ioctl.h>
#include <time.h>
#include <unistd.h>
#include <linux/vfio.h>
#include <immintrin.h>

#define VFIO_GROUP_PATH "/dev/vfio/16"
#define VFIO_CONTAINER_PATH "/dev/vfio/vfio"
#define PCI_DEVICE_NAME "0000:22:00.0"
#define MMAP_SIZE 0x5000 // Need at least 0x4000 + 64 bytes
#define BYTES_TO_PRINT 64

#define PASSERT(condition, message, label)                          \
    do {                                                            \
        if (!(condition))                                           \
        {                                                           \
            fprintf(stderr, "%s:%d: %s (%s)\n",                     \
                    __FILE__, __LINE__, message, strerror(errno));  \
            status = errno;                                         \
            goto label;                                             \
        }                                                           \
    }                                                               \
    while (0)

uint64_t get_time_ns() {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ULL + ts.tv_nsec;
}

void print_hex_bytes(const char *label, void *ptr, size_t count)
{
    unsigned char *bytes = (unsigned char *)ptr;
    printf("%s:\n", label);

    for (size_t i = 0; i < count; i++)
    {
        if (i % 16 == 0)
        {
            printf("%04lx: ", i);
        }
        printf("%02x ", bytes[i]);
        if ((i + 1) % 16 == 0)
        {
            printf("\n");
        }
    }
    if (count % 16 != 0)
    {
        printf("\n");
    }
    printf("\n");
}

int main()
{
    int container_fd, group_fd, device_fd, status = 0;
    struct vfio_group_status group_status = {.argsz = sizeof(group_status)};
    struct vfio_device_info device_info = {.argsz = sizeof(device_info)};
    struct vfio_region_info region_info = {.argsz = sizeof(region_info)};
    void *mapped_mem;

    // Open VFIO container
    container_fd = open(VFIO_CONTAINER_PATH, O_RDWR);
    PASSERT(container_fd > 0, "Error opening VFIO container", err_exit);

    // Check VFIO API version
    PASSERT(ioctl(container_fd, VFIO_GET_API_VERSION) == VFIO_API_VERSION,
            "Error getting VFIO API version", err_container);

    // Check if container supports VFIO_TYPE1_IOMMU
    PASSERT(ioctl(container_fd, VFIO_CHECK_EXTENSION, VFIO_TYPE1_IOMMU),
            "Container doesn't support VFIO_TYPE1_IOMMU", err_container);

    // Open VFIO group
    group_fd = open(VFIO_GROUP_PATH, O_RDWR);
    PASSERT(group_fd, "Error opening VFIO group", err_container);

    // Check group status
    PASSERT(ioctl(group_fd, VFIO_GROUP_GET_STATUS, &group_status) == 0,
            "Error getting group status", err_group);
    PASSERT(group_status.flags & VFIO_GROUP_FLAGS_VIABLE,
            "Group is not viable", err_group);

    // Set container for group
    PASSERT(ioctl(group_fd, VFIO_GROUP_SET_CONTAINER, &container_fd) == 0,
            "Error setting container for group", err_group);

    // Enable IOMMU model
    PASSERT(ioctl(container_fd, VFIO_SET_IOMMU, VFIO_TYPE1_IOMMU) == 0,
            "Error setting IOMMU for group", err_group);

    // Get device file descriptor
    device_fd = ioctl(group_fd, VFIO_GROUP_GET_DEVICE_FD, PCI_DEVICE_NAME);
    PASSERT(device_fd, "Error getting device FD", err_group);

    // Get device info
    PASSERT(ioctl(device_fd, VFIO_DEVICE_GET_INFO, &device_info) == 0,
            "Error getting device info", err_device);

    printf("Device has %d regions and %d IRQs\n", device_info.num_regions, device_info.num_irqs);

    // Get region 0 (BAR0) info
    region_info.index = 0;
    PASSERT(ioctl(device_fd, VFIO_DEVICE_GET_REGION_INFO, &region_info) == 0,
            "Error getting region info", err_device);

    printf("Region 0: size=0x%llx, offset=0x%llx, flags=0x%x\n",
           region_info.size, region_info.offset, region_info.flags);

    // Check if region 0 is mappable
    PASSERT(region_info.flags & VFIO_REGION_INFO_FLAG_MMAP,
            "Region 0 is not mappable", err_device);

    // Map the device memory
    mapped_mem = mmap(NULL, MMAP_SIZE, PROT_READ | PROT_WRITE, MAP_SHARED, device_fd, region_info.offset);
    PASSERT(mapped_mem != MAP_FAILED, "Error mmapping device memory", err_device);

    printf("Successfully mapped PCI device 22:00.0 via VFIO\n");
    printf("Mapped size: 0x%x bytes\n\n", MMAP_SIZE);

    uint8_t buffer[4096];
    uint64_t start_time = get_time_ns();
    for (int i = 0; i < 1024; i++) {
        // Use AVX2 intrinsics for memcpy (assume buffer and mapped_mem are 32-byte aligned)
#ifdef __AVX2__
        __m256i* dst = (__m256i*)buffer;
        __m256i* src = (__m256i*)mapped_mem;
        for (int j = 0; j < 4096 / 32; j++) {
            __m256i val = _mm256_load_si256(&src[j]);
            _mm256_store_si256(&dst[j], val);
        }
#else
        memcpy(buffer, mapped_mem, 4096);
#endif
    }
    uint64_t end_time = get_time_ns();

    printf("Benchmark\n");
    printf("--------------------------------\n");
    printf("H2D: Time to read 4MiB: %.3f ms\n", (end_time - start_time) / 1000000.0);

    float time_to_copy_s = (end_time - start_time) / 1000000000.0;
    printf("H2D read bandwidth: %.3f MiB/s\n", 4.0 / time_to_copy_s);
    printf("--------------------------------\n\n");

    for (int i = 0; i < 4096; i++) {
        buffer[i] = i % 256;
    }

    start_time = get_time_ns();
    for (int i = 0; i < 4; i++) {
        memcpy(mapped_mem, buffer, 4096);
    }
    end_time = get_time_ns();
    printf("H2D: Time to write 16kiB: %.3f ms\n", (end_time - start_time) / 1000000.0);

    time_to_copy_s = (end_time - start_time) / 1000000000.0;
    printf("H2D write bandwidth: %.3f MiB/s\n", 4.0 * 4 / 1024.0 / time_to_copy_s);
    printf("--------------------------------\n\n");

    // Print first 64 bytes at offset 0
    print_hex_bytes("First 64 bytes at offset 0", mapped_mem, BYTES_TO_PRINT);
    *(uint64_t*)mapped_mem = 0x1234567890abcdef;
    print_hex_bytes("First 64 bytes at offset 0 after write", mapped_mem, BYTES_TO_PRINT);

    // Print 64 bytes at offset 0x4000
    print_hex_bytes("64 bytes at offset 0x4000", (uint8_t*)mapped_mem + 0x4000, BYTES_TO_PRINT);
    *(uint64_t*)(mapped_mem + 0x4000) = 0x1234567890abcdef;
    print_hex_bytes("64 bytes at offset 0x4000 after write", (uint8_t*)mapped_mem + 0x4000, BYTES_TO_PRINT);

    // Cleanup
    PASSERT(munmap(mapped_mem, MMAP_SIZE) == 0, "Error unmapping memory", err_device);

err_device:
    close(device_fd);
err_group:
    close(group_fd);
err_container:
    close(container_fd);
err_exit:
    printf("Memory unmapped and all file descriptors closed successfully\n");
    return status;
}
