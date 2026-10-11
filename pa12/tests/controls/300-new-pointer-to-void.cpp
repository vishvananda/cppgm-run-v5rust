// N3485 5.3.4/1-3: in new void*(p) the new-type-id is void* (a complete object
// type), not void; expect exit 0
int main() { int x = 1; void** p = new void*(&x); bool ok = *p == &x; delete p; return ok ? 0 : 1; }
