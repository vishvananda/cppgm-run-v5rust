template<class Derived>
struct machine {
  template<int State, class CharacterClass, int NextState,
           void (Derived::*Action)(char)>
  struct row {
    static void execute(Derived& d, char event) {
      (d.*Action)(event);
    }
  };

protected:
  void push(char) {}
};

#define FSM(Fsm) \
  template<class Ch> \
  void push(Ch c) { machine<Fsm>::push(c); }

struct f : machine<f> {
  FSM(f)

  typedef f self;

  void on(char) {
    push('x');
  }

  typedef row<0, int, 0, &self::on> first;
  typedef row<0, int, 0, &self::push> second;
};

int main() {
  f x;
  f::first::execute(x, 0);
  f::second::execute(x, 0);
  return 0;
}
