// N3485 14.8.2p8: missing, non-type, scalar, inaccessible and ambiguous
// member types fail signature substitution without demanding bodies; expect exit 0
struct missing {};
struct value { static int type; };
class hidden { typedef int type; };
struct left { typedef int type; };
struct right { typedef short type; };
struct ambiguous : left, right {};
template<class T> typename T::type choose(T) { return T::invalid_body; }
long choose(...);
struct incomplete;
template<class T> typename T::type probe(T*);
long probe(...);
int first = sizeof(probe((incomplete*)0));
struct incomplete { typedef int type; };
int main() {
  return sizeof(choose(missing())) == sizeof(long)
      && sizeof(choose(value())) == sizeof(long)
      && sizeof(choose(hidden())) == sizeof(long)
      && sizeof(choose(ambiguous())) == sizeof(long)
      && sizeof(choose(3)) == sizeof(long)
      && sizeof(probe((incomplete*)0)) == sizeof(int) ? 0 : 1;
}
