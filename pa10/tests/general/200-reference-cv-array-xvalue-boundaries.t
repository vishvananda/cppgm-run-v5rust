// Consolidated successful boundaries; each original check remains independent.

// 200-reference-cv-array-const-xvalue
namespace case_0 {
// C++11 reference initialization: N3485 8.5.3.
int check_case(){const int a[2]={1,2};const int(&r)[2]=static_cast<const int(&&)[2]>(a);return &r[0]!=&a[0]||r[1]!=2;}

}

// 200-reference-cv-array-multidim-const-xvalue
namespace case_1 {
// C++11 reference initialization: N3485 8.5.3.
int check_case(){const int a[2][2]={{1,2},{3,4}};const int(&r)[2][2]=static_cast<const int(&&)[2][2]>(a);return &r[0][0]!=&a[0][0]||r[1][1]!=4;}

}

// 200-reference-cv-array-mutable-xvalue-const-reference
namespace case_2 {
// C++11 reference initialization: N3485 8.5.3.
int check_case(){int a[2]={1,2};const int(&r)[2]=static_cast<int(&&)[2]>(a);return &r[0]!=&a[0]||r[1]!=2;}

}

// 200-reference-cv-volatile-array-rvalue
namespace case_3 {
// C++11 array qualification: N3485 3.9.3/5, 4.4/4 and 8.5.3.
int check_case(){int a[2]={1,2};volatile int(&&r)[2]=static_cast<int(&&)[2]>(a);r[1]=5;return a[1]-5;}

}

int main() {
  if (case_0::check_case()) return 1;
  if (case_1::check_case()) return 2;
  if (case_2::check_case()) return 3;
  if (case_3::check_case()) return 4;
  return 0;
}
