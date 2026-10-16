// N3485 12.8: ordinary use still rejects a deleted defaulted copy; expect reject
struct NoCopy { NoCopy(const NoCopy&) = delete; };
template<class T> struct Box { T value; Box(const Box&) = default; };
template struct Box<NoCopy>;
void copy(const Box<NoCopy>& source) { Box<NoCopy> target(source); }
