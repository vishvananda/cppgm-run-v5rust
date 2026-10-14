// expect reject
int main() {
    goto absent;
first:
    return 0;
first:
    return 1;
}
