// expect exit 0
int x;
int count;
int *address() {
    ++count;
    return &x;
}
int main() {
    *address();
    (void)*address();
    (void)x;
    volatile int *p = &x;
    *p;
    (void)*p;
    return count != 2;
}
