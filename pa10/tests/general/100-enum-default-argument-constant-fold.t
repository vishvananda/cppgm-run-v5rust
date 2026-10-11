enum E { A };

int f(E x = A) { return x == A; }

int g() { return f(); }

int main() { return g(); }
