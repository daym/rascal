unit u;
interface
type
  tarr = array[0..1, 0..1, 0..3] of byte;
const
  table : tarr = (
    ((1, 2, 3, 4), (5, 6, 7, 8)),
    ((9, 10, 11, 12), (13, 14, 15, 16))
  );
implementation
end.
