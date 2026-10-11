template<typename T>
int pick(T * p) { return 1; }

int pick(int x) { return 2; }

int main() { int x = 0; return pick(&x) - 1; }
