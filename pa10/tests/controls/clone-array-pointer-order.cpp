// expect exit 0
int data[6] = {1, 2, 3, 4, 5, 6};
int calls;
int *base() { calls = calls * 10 + 1; return data; }
int offset() { calls = calls * 10 + 2; return 2; }
int main() {
    int value = offset()[base()];
    if (value != 3 || calls != 12) return 1;
    int *p = data;
    p += 3;
    --p;
    int old = *p++;
    p -= 1;
    return old != 3 || p - data != 2 || 1[p] != 4 || *(data + 4) != 5;
}
