namespace api {
template<class T>
struct collate {
  T value;
};

namespace regex_constants {
int collate;
}
}

using namespace api;

template<class T>
int use_facet(T *) {
  return sizeof(T);
}

int main() {
  (void)api::regex_constants::collate;
  return use_facet<collate<int> >(0) == sizeof(collate<int>) ? 0 : 1;
}
