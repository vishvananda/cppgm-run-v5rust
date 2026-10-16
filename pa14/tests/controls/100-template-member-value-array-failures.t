// expect exit 0
// Invalid bounds drop candidates; a failed incomplete lookup does not poison retry.
struct hidden { private: static const int count = 3; };
struct zero { static const int count = 0; };
struct negative { static const int count = -1; };
struct type_name { using count = int; };
struct missing {};
template<class T> char select(T*, int (&)[T::count]);
long select(void*, const void*);
struct late;
int before = sizeof(select((late*)0, *(int (*)[3])0));
struct late { static const int count = 3; };
int main() {
  int values[3];
  return sizeof(select((hidden*)0, values)) == sizeof(long)
      && sizeof(select((zero*)0, values)) == sizeof(long)
      && sizeof(select((negative*)0, values)) == sizeof(long)
      && sizeof(select((type_name*)0, values)) == sizeof(long)
      && sizeof(select((missing*)0, values)) == sizeof(long)
      && before == sizeof(long) && sizeof(select((late*)0, values)) == 1 ? 0 : 1;
}
