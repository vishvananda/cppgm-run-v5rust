template<typename Derived, typename Ch = char>
struct machine {
  typedef Ch char_type;

  template<Ch C>
  struct is {};

  template<int State, typename CharacterClass, int NextState,
           void (Derived::*Action)(char_type)>
  struct row {};
};

struct unix2dos_fsm : machine<unix2dos_fsm> {
  typedef unix2dos_fsm self;

  void on_lf(char) {}

  typedef row<0, is<'\n'>, 0, &self::on_lf> transition_table;
};

int main() {
  unix2dos_fsm fsm;
  (void) fsm;
  return 0;
}
