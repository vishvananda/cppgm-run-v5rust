// N3485 5.2.10p2: reinterpret_cast shall not cast away constness; must be
// rejected
int main() { const int i = 0; const int* p = &i; int* a = reinterpret_cast<int*>(p); return *a; }
