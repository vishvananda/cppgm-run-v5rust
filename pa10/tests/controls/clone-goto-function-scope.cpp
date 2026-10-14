// expect reject
int other() {
target:
    return 1;
}
int main() {
    goto target;
}
