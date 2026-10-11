template<class Derived>
struct machine {
  template<int State, class CharacterClass, int NextState,
           void (Derived::*Action)(char)>
  struct row {
    static void execute(Derived& d, char event) {
      (d.*Action)(event);
    }
  };
};

struct f : machine<f> {
  typedef f self;

  int value;

  f() : value(0) {}

  void on(char event) {
    value = event;
  }

  typedef row<0, int, 0, &self::on> transition;
};

int main() {
  f x;
  f::transition::execute(x, 7);
  return x.value == 7 ? 0 : 1;
}
