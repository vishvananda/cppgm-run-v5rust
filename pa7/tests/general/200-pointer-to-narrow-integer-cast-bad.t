// N3485 5.2.10p4: a pointer converts only to an integral type large enough to
// hold it; (int)p is ill-formed; must be rejected
int use(int* p) { return (int)p; }
int main() { return use(0); }
