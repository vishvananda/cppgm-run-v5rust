int destroyed;
struct T { volatile int* p; ~T() noexcept {++destroyed;} };
void selected(bool b,volatile int& v){b?T{&v}.p[0]:v;}
void nested(bool b,volatile int& v){b?(T{&v}.p[0],v):v;}
int main(){
  volatile int value=7;
  selected(false,value);
  if(destroyed!=0)return 1;
  selected(true,value);
  if(destroyed!=1)return 2;
  nested(false,value);nested(true,value);
  return destroyed==2?0:3;
}
