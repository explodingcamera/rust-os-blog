MEMORY
{
  /* Linker space for the boot image, not the machine's total RAM. */
  RAM : ORIGIN = 0x80200000, LENGTH = 16M
}

REGION_ALIAS("REGION_TEXT", RAM);
REGION_ALIAS("REGION_RODATA", RAM);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM);
REGION_ALIAS("REGION_STACK", RAM);

/* riscv-rt reserves .heap using _heap_size. Leave a page on each side. */
_heap_size = 128K + 12K; /* Includes alignment space before the first guard. */
_heap_guard_start = ALIGN(__sheap, 4K);
_heap_start = _heap_guard_start + 4K;
_heap_guard_end = _heap_start + 128K;

/* Leave an unmapped page below the 16 KiB boot stack. */
_stack_guard_start = ALIGN(__estack, 4K);
_stack_start = _stack_guard_start + 4K + 16K;

ASSERT(_heap_guard_end + 4K <= __eheap, "boot heap guard exceeds reserved heap");
