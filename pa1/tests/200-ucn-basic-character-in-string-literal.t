// N3485 2.3p2: a universal-character-name naming a basic source character is
// ill-formed only outside character and string literals; "\u0041\u0022" is a
// valid string literal (full program: exit 0)
int main() { const char* s = "\u0041\u0022"; return s[0] == 0x41 && s[1] == 0x22 ? 0 : 1; } // 0
