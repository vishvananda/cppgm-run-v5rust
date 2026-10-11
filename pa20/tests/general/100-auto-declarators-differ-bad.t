// N3485 7.1.6.4p7: the type deduced for auto must be the same in every
// declarator (int vs double); must be rejected
int main() { auto a = 5, b = 6.0; return a + (int)b; }
