#include <malloc/malloc.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <stddef.h>
#include <mach/mach.h>
static void snap(const char *label) {
  malloc_statistics_t stats;
  malloc_zone_statistics(NULL, &stats);
  printf("%s in_use=%zu allocated=%zu blocks=%u\n", label, stats.size_in_use, stats.size_allocated, stats.blocks_in_use);
  task_vm_info_data_t vm = {0};
  mach_msg_type_number_t count = TASK_VM_INFO_COUNT;
  kern_return_t status = task_info(mach_task_self(), TASK_VM_INFO, (task_info_t)&vm, &count);
  printf("vm_status=%d resident=%llu footprint=%llu reusable=%llu internal=%llu compressed=%llu\n", status, vm.resident_size, vm.phys_footprint, vm.reusable, vm.internal, vm.compressed);
  fflush(stdout);
  char cmd[128];
  snprintf(cmd, sizeof(cmd), "ps -o rss= -p %d", getpid());
  system(cmd);
}
int main(void) {
  printf("ABI malloc_size=%zu vm_rev1_words=%u reusable_offset=%zu footprint_offset=%zu\n", sizeof(malloc_statistics_t), (unsigned)TASK_VM_INFO_REV1_COUNT, offsetof(task_vm_info_data_t,reusable), offsetof(task_vm_info_data_t,phys_footprint));
  snap("start");
  void *p[8];
  for (int i=0;i<8;i++) {p[i] = malloc(32*1024*1024); memset(p[i], 91+i, 32*1024*1024); __asm__ volatile("" : : "r"(p[i]) : "memory");}
  snap("allocated_256MiB");
  for (int i=0;i<8;i++) free(p[i]);
  snap("freed");
  usleep(100000);
  snap("freed_100ms");
  if (getenv("PANEL_Y_VM_MAP")) {
    malloc_zone_print(NULL, true);
    char command[128];
    snprintf(command, sizeof(command), "/usr/bin/vmmap -summary %d", getpid());
    system(command);
  }
  size_t relieved = malloc_zone_pressure_relief(NULL, 0);
  printf("pressure_relief_bytes=%zu\n", relieved);
  snap("relieved");
  for (int round=0;round<3;round++) {
    for (int i=0;i<8;i++) {p[i] = malloc(32*1024*1024); memset(p[i], 93+i+round, 32*1024*1024); __asm__ volatile("" : : "r"(p[i]) : "memory");}
    snap("reused_256MiB");
    for (int i=0;i<8;i++) free(p[i]);
    snap("refreed");
  }
}
