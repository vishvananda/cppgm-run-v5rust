// expect exit 0
int main() {
    volatile char a[4] = {("hi")};
    volatile wchar_t b[4] = {(L"hi")};
    volatile char rows[2][4] = {("hi"), ("bye")};
    return a[0] != 'h' || a[1] != 'i' || a[2] != 0 || a[3] != 0
        || b[1] != L'i' || b[3] != 0 || rows[1][2] != 'e' || rows[1][3] != 0;
}
