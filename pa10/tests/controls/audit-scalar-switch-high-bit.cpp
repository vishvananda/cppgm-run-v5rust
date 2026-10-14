// expect exit 0
int signed_case(int x) {
    switch (x) {
    case -1: return 0;
    default: return 1;
    }
}
int main() {
    unsigned long x = 18446744073709551615ul;
    switch (x) {
    case 18446744073709551615ul: return signed_case(-1);
    default: return 1;
    }
}
