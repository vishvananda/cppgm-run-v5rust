// N3485 5p6, 5.16: the projected conversion result keeps its expression type.
struct Value {
  static const bool value = true;
  operator const bool() { return value; }
};
struct Reference {
  static const bool value = false;
  operator const bool&() { return value; }
};
int from_value() { return Value() ? 1 : 2; }
int from_reference(Reference& r) { return r ? 3 : 4; }
