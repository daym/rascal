unit u;
interface
type
  tdefoption = (do_one, do_two, do_three);
  tindex = 1..ord(high(tdefoption));
  tmap = array[tindex] of byte;
implementation
end.
