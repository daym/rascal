unit u;
interface
procedure run;
implementation
procedure run;
var
  n : record
    n_un : record
      case longint of
        0 : (n_name : pchar);
        1 : (n_strx : longint);
    end;
  end;
begin
  n.n_un.n_strx := 7;
end;
end.
