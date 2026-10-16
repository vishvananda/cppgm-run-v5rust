// N3485 12.4, 14.7.2: explicit destructor emission checks subobject deletion; expect reject
struct NoDestroy { ~NoDestroy() = delete; };
template<class T> struct Box { T value; ~Box() = default; };
template struct Box<NoDestroy>;
