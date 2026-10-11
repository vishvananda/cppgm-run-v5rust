// N3485 8.5.2p1: an array of char, signed char or unsigned char may be
// initialized by a narrow string literal (full program: exit 11)
signed char sc[] = "abc"; unsigned char uc[4] = "abc"; signed char sd[] = {"ab"};
int main() { return sizeof(sc) + sizeof(uc) + sizeof(sd) + (sc[1] == 'b' ? 0 : 50) + (uc[2] == 'c' ? 0 : 60); } // 11
