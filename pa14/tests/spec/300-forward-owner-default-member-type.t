// VALIDATION: compile-pass
// N3485 focus: 14.1 [temp.param], 14.7.1 [temp.inst]

template<class T>
class Alloc;

template<class CharT, class AllocT = Alloc<CharT> >
struct String;

typedef String<int> IntString;

template<class T>
class Alloc {
public:
  typedef T value_type;
};

template<class AllocT, class = typename AllocT::value_type>
struct AllocTraits {
  typedef AllocT allocator_type;
};

template<class CharT, class AllocT>
struct String {
  typedef AllocTraits<AllocT> traits_type;
};

struct Holder {
  IntString value;
};

int main()
{
  Holder h;
  (void)h;
  return 0;
}
