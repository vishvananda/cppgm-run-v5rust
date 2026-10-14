// expect reject
int main() {
    goto target;
    int value = 3;
    {
    target:
        return value;
    }
}
