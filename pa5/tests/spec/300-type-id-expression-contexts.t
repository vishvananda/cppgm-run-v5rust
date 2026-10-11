// N3485 focus: 8.1 [dcl.name] type-id syntax in expression contexts
int f(int *p) {
  return sizeof(int*) + static_cast<int>(*p);
}
