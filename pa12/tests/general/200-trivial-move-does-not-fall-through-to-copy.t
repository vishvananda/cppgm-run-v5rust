struct base { int n; };

struct value : base {
  value() {}
  value(value const & other) : base(other) {}
  value(value &&) = default;
};

value && forward(value & v) { return static_cast<value &&>(v); }
void take(value) {}
void copy(value const & v) { value copied(v); }

int main() {
  value v;
  copy(v);
  take(forward(v));
}
