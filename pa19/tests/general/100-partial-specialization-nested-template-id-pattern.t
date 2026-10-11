// N3485 focus: [temp.class.spec.match] matching class partial specializations.
namespace ns {

template<class T>
struct trait { typedef T type; };

template<class T, class Traits = trait<T>, class Alloc = int>
struct string_like {};

typedef string_like<char> string_alias;

template<class T>
struct hash { enum { value = 1 }; };

template<class Alloc>
struct hash<string_like<char, trait<char>, Alloc> > {
  enum { value = 7 };
};

}

int main() {
  return ns::hash<ns::string_alias>::value == 7 ? 0 : 1;
}
