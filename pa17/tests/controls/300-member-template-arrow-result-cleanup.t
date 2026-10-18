// expect exit 0
// Implicit operator-> call results keep their member class-template owner
// in the full-expression cleanup query, just as explicit calls do.
int destructions;
struct Guard {
  ~Guard() noexcept { ++destructions; }
};
struct Item {
  int value;
  void run() noexcept { ++value; }
};
struct Owner {
  template<class T> struct Proxy {
    T* pointer;
    ~Proxy() noexcept { ++destructions; }
    T* operator->() noexcept { return pointer; }
  };
};
struct Source {
  Item* pointer;
  Owner::Proxy<Item> operator->() noexcept {
    return Owner::Proxy<Item>{pointer};
  }
};
Source get_source(Item* pointer) noexcept { return Source{pointer}; }
int observed;
void consume(int value) noexcept { observed = value; }
void exercise() {
  Guard outer;
  Item item{7};
  Source source{&item};
  source->run();
  source->value;
  int value = source->value;
  consume(source->value);
  (source->value, source->value);
  if (value != 8 || observed != 8) destructions += 100;
}
int main() {
  exercise();
  Item item{0};
  // The inert Source result ends the initial region. With no cleanup alive,
  // the next nonthrowing call does not reopen an empty region.
  get_source(&item)->run();
  return destructions != 8;
}
