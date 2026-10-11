// N3485 8.5.1p6: too many initializer-clauses for the array of an array
// new-expression is ill-formed; must be rejected
void use() { new int[2]{1, 2, 3}; }
int main() { return 0; }
