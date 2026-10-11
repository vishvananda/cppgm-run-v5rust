// N3485 8.5.4p7: unsigned char c{n} with const int n = 300 narrows (the value
// does not fit); must be rejected
int main() { const int n = 300; unsigned char c{n}; return c; }
