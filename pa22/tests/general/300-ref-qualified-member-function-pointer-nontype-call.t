struct object { bool read() const & { return true; } };

template<class Class, class Result, class Pointer, Pointer Member>
struct extractor {
  Result operator()(const Class& value) const { return (value.*Member)(); }
};

extractor<object, bool, bool (object::*)() const &, &object::read> value;

int main() {
  object input;
  return value(input) ? 0 : 1;
}
