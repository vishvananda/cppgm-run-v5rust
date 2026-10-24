struct C { int call(int); };
int invalid() {
  return __builtin_invoke(&C::call);
}
