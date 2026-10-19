// N3485 15.1p3: the operand of throw cannot have type void; expect reject
void g();
void f() { throw g(); }
int main() { return 0; }
