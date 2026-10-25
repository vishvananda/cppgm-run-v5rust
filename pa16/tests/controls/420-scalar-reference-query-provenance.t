// N3485 3.6.2p2, 7.1.5, 12.2p5: reference fallback retains its evaluated
// temporary's identity and writable unqualified storage; expect exit 0
constexpr const double& empty_ref=double{};
constexpr const double& one_ref=double{1.};
constexpr double two(){return 2.;}
constexpr const double& call_ref=two();
constexpr int integer_two(){return 2;}
constexpr const double& converted_ref=double(integer_two());
typedef double Number;
constexpr const Number& alias_ref=Number{};
extern const double& later_ref;
const double* early_ptr=&later_ref;
constexpr const double& later_ref=double();
static_assert(empty_ref==0. && one_ref==1. && call_ref==2.,"values");
static_assert(converted_ref==2. && alias_ref==0. && later_ref==0.,"aliases");
int main(){
 const_cast<double&>(empty_ref)=3.;
 return empty_ref==3. && one_ref==1. && call_ref==2. && converted_ref==2.
  && alias_ref==0. && *early_ptr==0. ? 0 : 1;
}
