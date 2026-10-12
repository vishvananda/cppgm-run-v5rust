template<class T> struct P{template<class U>struct Nested{};};
int * _Nonnull scalar;
int * _Nullable const cv;
int P<int>::* _Null_unspecified member;
int (P<int>::* _Nullable_result method)(int);
int P<int>::template Nested<int>::* nested;
