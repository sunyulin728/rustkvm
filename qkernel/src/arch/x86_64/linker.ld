ENTRY(_start)

SECTIONS {
    . = 0x7048000000;

    .text :
    {
        *(.text)
    }

    .data :
    {
        *(.data)
    }

}
