namespace types {
struct value {};
}

::types::value* pointer;

struct holder {
  operator ::types::value*() const;
};
