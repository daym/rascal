unit u;
interface
type
  titem = record
    value : integer;
  end;
  titems = array[word] of titem;
  tbox = class
    procedure fill(var items : titems);
  end;
implementation
end.
