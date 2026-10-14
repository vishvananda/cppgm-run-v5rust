// expect exit 0
int main() {
    sizeof(int);
    alignof(int);
    return (sizeof(int) + 1 != 5) + (1 + sizeof(int) != 5)
        + (sizeof(int) - 1 != 3) + (alignof(int) + 1 != 5);
}
