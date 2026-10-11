// N3485 2.3p2: a universal-character-name naming a control character is
// ill-formed only outside character and string literals; '\u000A' is a valid
// character literal (full program: exit 10)
int main() { char c = '\u000A'; return c; } // 10
