enum __typeof__(int) { number = 3 } value;
int main() { return number - 3 + sizeof(value) - 4; }
