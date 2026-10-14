// expect reject
// Implicit copy/move candidates exist but cannot take a scalar list element.
struct Object { int value = 1; };
int main() { Object object{2}; return object.value; }
