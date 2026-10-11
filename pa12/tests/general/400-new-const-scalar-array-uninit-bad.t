// N3485 8.5p6, 5.3.4p15: new const int[n] default-initializes const objects of
// non-class type: ill-formed; must be rejected
int main() { int n = 2; const int* p = new const int[n]; return p != 0 ? 0 : 1; }
