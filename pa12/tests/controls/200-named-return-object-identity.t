// A return statement constructs its operand directly in the caller's result
// object only when the operand names a non-volatile automatic object of the
// result's class type (N3485 12.8p31). A local reference names another
// object, which is copied into the result and still destroyed by its own
// scope.
int destroyed = 0;

struct Tracked {
  int value;
  Tracked(int v) : value(v) {}
  Tracked(const Tracked& other) : value(other.value + 100) {}
  ~Tracked() { ++destroyed; }
};

Tracked through_reference(int v) {
  Tracked local(v);
  Tracked& alias = local;
  return alias;
}

Tracked named_object(int v) {
  Tracked local(v);
  return local;
}

int main() {
  int status = 0;
  {
    Tracked copied = through_reference(5);
    if (copied.value != 105) status |= 1;
    if (destroyed != 1) status |= 2;
  }
  if (destroyed != 2) status |= 4;
  {
    Tracked named = named_object(7);
    if (named.value != 7 && named.value != 107) status |= 8;
  }
  return status;
}
