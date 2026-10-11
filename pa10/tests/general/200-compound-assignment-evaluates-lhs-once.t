int count, value;
int &lhs() { ++count; return value; }
int main() { lhs() += 2; return count; }
