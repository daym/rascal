unit u;
interface
type
  TNode = class end;
  TTempCreateNode = class;
const
  StoreFlags = 1;
type
  PTempInfo = ^TTempInfo;
  TTempInfo = record
    Owner : TTempCreateNode;
  end;
  TTempCreateNode = class(TNode)
    TempInfo : PTempInfo;
  end;
implementation
end.
