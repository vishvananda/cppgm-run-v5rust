enum E { high = 4294967295U };
int which(int) { return 1; }
int which(unsigned) { return 0; }
int main() { return which(high); }
