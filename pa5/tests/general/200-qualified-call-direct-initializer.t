// Qualified calls in a direct initializer must not become parameter types.
namespace Values {
typedef int Number;
int fetch(int value);
template<class T> T forward(T value);
}
struct Tag {};
struct Record { Record(Tag, int, int); };
template<class T>
struct Operations { static T fetch(T value); };

template<class T>
void construct(T first, T last)
{
  const Record direct(Tag(), Values::fetch(first), ::Values::forward(last));
  const Record qualified(Tag(), Operations<T>::fetch(first), Values::forward<T>(last));
}

// Qualified types still allow parenthesized names and function parameters.
int named(Values::Number(value));
Record declaration(Tag(), Values::Number(), int());
