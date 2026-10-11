// Consolidated successful boundaries; each original check remains independent.

// 100-auto-const-reference-class-aggregate
namespace case_0 {
struct S{int x;};int check_case(){const auto& s=S{3};return s.x!=3;}

}

// 100-auto-const-reference-class-borrowed-call
namespace case_1 {
int alive;struct S{int x;S(int v)noexcept:x(v){++alive;}~S()noexcept{--alive;}};S& borrow(S&v)noexcept{return v;}int check_case(){S value(3);{const auto& s=borrow(value);if(&s!=&value||alive!=1)return 1;}return alive!=1;}

}

// 100-auto-const-reference-class-call
namespace case_2 {
int alive;struct S{int x;S(int v)noexcept:x(v){++alive;}S(const S&s)noexcept:x(s.x){++alive;}~S()noexcept{--alive;}};S get()noexcept{return S(3);}int check_case(){{const auto& s=get();if(alive!=1||s.x!=3)return 1;}return alive!=0;}

}

// 100-auto-const-reference-class-conditional
namespace case_3 {
int alive,pick;struct S{int x;S(int v)noexcept:x(v){++alive;}S(const S&s)noexcept:x(s.x){++alive;}~S()noexcept{--alive;}};int check_case(){for(int p=0;p<2;++p){pick=p;{const auto& s=pick?S(3):S(4);if(alive!=1||s.x!=(pick?3:4))return 1;}if(alive)return 2;}return 0;}

}

// 100-auto-const-reference-class-const-rvalue-auto
namespace case_4 {
struct S{int x;};const S get(){return S{3};}int check_case(){auto& s=get();return s.x!=3;}

}

// 100-auto-const-reference-class-runtime
namespace case_5 {
int calls;struct S{int x;S(int v):x(v){++calls;}};int check_case(){const auto& s=S(3);return s.x!=3||calls!=1;}

}

// 100-auto-const-reference-global-reference-runtime
namespace case_6 {
int calls;struct S{int x;S(int v):x(v){++calls;}};const auto& s=S(3);int check_case(){return s.x!=3||calls!=1;}

}

// 100-auto-const-reference-scalar-array-reference
namespace case_7 {
int check_case(){const auto& text="ab";return sizeof(text)!=3||text[1]!='b';}

}

// 100-auto-const-reference-scalar-function-reference
namespace case_8 {
int f(){return 3;}int check_case(){const auto& function=f;return function()!=3;}

}

// 100-auto-const-reference-scalar-runtime
namespace case_9 {
int calls;int get(){++calls;return 3;}int check_case(){const auto& x=get();return x!=3||calls!=1;}

}

// 100-auto-const-reference-scalar-rvalue
namespace case_10 {
int check_case(){const auto& x=3;return x!=3;}

}

// 100-auto-const-reference-static-reference-runtime
namespace case_11 {
int calls;struct S{int x;S(int v):x(v){++calls;}};const S& get(){static const auto& s=S(3);return s;}int check_case(){const S& s=get();return s.x!=3||calls!=1||&s!=&get();}

}

int main() {
  if (case_0::check_case()) return 1;
  if (case_1::check_case()) return 2;
  if (case_2::check_case()) return 3;
  if (case_3::check_case()) return 4;
  if (case_4::check_case()) return 5;
  if (case_5::check_case()) return 6;
  if (case_6::check_case()) return 7;
  if (case_7::check_case()) return 8;
  if (case_8::check_case()) return 9;
  if (case_9::check_case()) return 10;
  if (case_10::check_case()) return 11;
  if (case_11::check_case()) return 12;
  return 0;
}
