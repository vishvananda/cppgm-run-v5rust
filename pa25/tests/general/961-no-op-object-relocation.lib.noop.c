__asm__(".data\n"
        ".globl link_note\n"
        ".type link_note,@object\n"
        "link_note:\n"
        ".quad 9\n"
        ".reloc link_note, R_X86_64_NONE\n"
        ".size link_note,8\n");
