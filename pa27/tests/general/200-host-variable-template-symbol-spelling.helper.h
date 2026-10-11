// A variable template specialization is one object across units only when
// both name it alike (`_Z5widthIiE`, `_ZN5store5countIcEE`).
template<class T> int width = int(sizeof(T));
namespace store { template<class T> int count = 0; }
