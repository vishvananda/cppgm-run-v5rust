template<class T>
struct box {};

struct owner {
  template<class U>
  box<box<U>> operator=(U&&);
};
