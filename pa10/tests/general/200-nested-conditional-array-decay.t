char short_array[3];
char first_array[4];
char second_array[4];
char third_array[4];

char* select_array(int index)
{
  return index == 0 ? short_array
                    : index == 1 ? first_array
                                 : index == 2 ? second_array : third_array;
}

int main()
{
  return select_array(1) == first_array ? 0 : 1;
}
