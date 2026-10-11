namespace ns {
inline namespace v1 {

template<class T>
struct state {};

template<class CharT>
class node {
public:
  typedef ns::state<CharT> state_type;
  virtual void exec(state_type&) const {}
};

template<class CharT>
class has_one_state : public node<CharT> {};

template<class CharT>
class match_any_but_newline : public has_one_state<CharT> {
public:
  typedef ns::state<CharT> state_type;
  void exec(state_type&) const override;
};

template<>
void match_any_but_newline<char>::exec(state_type&) const;

}  // namespace v1
}  // namespace ns

int main() {
  return 0;
}
