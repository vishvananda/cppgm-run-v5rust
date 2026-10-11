namespace std { inline namespace __1 {
template <class C>
class __owns_one_state {};

template <class C>
class __owns_two_states : public __owns_one_state<C> {
  typedef __owns_one_state<C> base;
  base* __second_;
public:
  __attribute__((__visibility__("hidden")))
  __attribute__((__exclude_from_explicit_instantiation__))
  virtual ~__owns_two_states();
};

template <class C>
__owns_two_states<C>::~__owns_two_states() {
  delete __second_;
}
}}
