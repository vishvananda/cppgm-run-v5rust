// N3485 14.2p4, PA17 README (dependent qualified types): typename T::R<double>
// needs the template keyword too (g++ lenient, clang rejects); expect reject
struct X { template<class U> struct R { }; };
template<class T> void g() { typename T::R<double> r; }
int main() { return 0; }
