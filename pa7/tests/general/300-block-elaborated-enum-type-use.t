enum E { e };
E id(E x) { return x; }
int main() { enum E x = e; return id(x); }
