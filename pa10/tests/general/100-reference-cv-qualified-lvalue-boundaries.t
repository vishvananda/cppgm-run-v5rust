// Consolidated successful boundaries; each original check remains independent.

// 100-reference-cv-array-const-volatile-lvalue
namespace case_0 {
// C++11 reference initialization: N3485 8.5.3.
int check_case(){volatile int a[2]={1,2};const volatile int(&r)[2]=a;return &r[0]!=&a[0]||r[1]!=2;}

}

// 100-reference-cv-array-overload
namespace case_1 {
// C++11 reference initialization: N3485 8.5.3.
int f(int(&)[2]){return 1;}int f(const int(&)[2]){return 2;}int check_case(){int a[2]={1,2};const int b[2]={3,4};return f(a)!=1||f(b)!=2;}

}

// 100-reference-cv-pointer-deep-const-reference-middle-qualified
namespace case_2 {
// C++11 reference initialization: N3485 8.5.3.
int check_case(){int a=3;int*p=&a;int**pp=&p;const int*const*const& r=pp;return **r!=3;}

}

// 100-reference-cv-scalar-const-volatile-lvalue
namespace case_3 {
// C++11 reference initialization: N3485 8.5.3.
int check_case(){volatile int a=3;const volatile int& r=a;return &r!=&a||r!=3;}

}

// 100-reference-cv-volatile-array-lvalue
namespace case_4 {
// C++11 array qualification: N3485 3.9.3/5, 4.4/4 and 8.5.3.
int check_case(){int a[2]={1,2};volatile int(&r)[2]=a;r[1]=5;return a[1]-5;}

}

// 100-reference-cv-volatile-array-multidim-lvalue
namespace case_5 {
// C++11 array qualification: N3485 3.9.3/5, 4.4/4 and 8.5.3.
int check_case(){int a[2][2]={{1,2},{3,4}};volatile int(&r)[2][2]=a;r[1][1]=5;return a[1][1]-5;}

}

// 100-reference-cv-volatile-array-pointer-deep-const
namespace case_6 {
// C++11 array qualification: N3485 3.9.3/5, 4.4/4 and 8.5.3.
int check_case(){int a[2]={1,2};int(*p)[2]=&a;int(**q)[2]=&p;volatile int(*const*r)[2]=q;(**r)[1]=5;return a[1]-5;}

}

// 100-reference-cv-volatile-array-pointer-reference
namespace case_7 {
// C++11 array qualification: N3485 3.9.3/5, 4.4/4 and 8.5.3.
int check_case(){int a=1;int b=2;int* p[2]={&a,&b};int*volatile(&r)[2]=p;r[1]=&a;return *p[1]-1;}

}

// 100-reference-cv-volatile-array-pointer
namespace case_8 {
// C++11 array qualification: N3485 3.9.3/5, 4.4/4 and 8.5.3.
int check_case(){int a[2]={1,2};int(*p)[2]=&a;volatile int(*q)[2]=p;(*q)[1]=5;return a[1]-5;}

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
  return 0;
}
