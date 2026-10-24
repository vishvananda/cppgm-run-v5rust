struct C { int value; };
int invalid(C &object) {
  return __builtin_invoke(&C::value, object, missing);
}
