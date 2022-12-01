#include <math.h>
#include "uav.h"
#include "main.h"

#include "ns3/udp-server.h"
#include "ns3/udp-client.h"
#include "ns3/udp-trace-client.h"
#include "ns3/string.h"
#include "ns3/uinteger.h"
#include "ns3/names.h"
#include "ns3/log.h"
#include "ns3/ipv4-address.h"
#include "ns3/address-utils.h"
#include "ns3/nstime.h"
#include "ns3/inet-socket-address.h"
#include "ns3/socket.h"
#include "ns3/udp-socket.h"
#include "ns3/simulator.h"
#include "ns3/socket-factory.h"
#include "ns3/packet.h"
#include "ns3/uinteger.h"
#include "ns3/mobility-model.h"
#include "ns3/waypoint-mobility-model.h"

using namespace ns3;

NS_LOG_COMPONENT_DEFINE ("UAV");

NS_OBJECT_ENSURE_REGISTERED (UAV);

TypeId
UAV::GetTypeId (void)
{
  static TypeId tid =
      TypeId ("HASH_POWS")
          .SetParent<Application> ()
          .SetGroupName ("Applications")
          .AddConstructor<UAV> ()
          .AddAttribute ("Port", "Port on which we listen for incoming packets.", UintegerValue (9),
                         MakeUintegerAccessor (&UAV::m_port),
                         MakeUintegerChecker<uint16_t> ())
          .AddTraceSource ("Rx", "A packet has been received",
                           MakeTraceSourceAccessor (&UAV::m_rxTrace),
                           "ns3::Packet::TracedCallback")
          .AddTraceSource ("RxWithAddresses", "A packet has been received",
                           MakeTraceSourceAccessor (&UAV::m_rxTraceWithAddresses),
                           "ns3::Packet::TwoAddressTracedCallback")
          .AddAttribute ("ServerAddress", "The address of the central server node", Ipv4AddressValue(Ipv4Address((uint32_t) 0)),
                           MakeIpv4AddressAccessor(&UAV::m_rootAddress),
                           MakeIpv4AddressChecker())
          .AddAttribute ("ClientAddress", "The address of the this uav", Ipv4AddressValue(Ipv4Address((uint32_t) 0)),
                           MakeIpv4AddressAccessor(&UAV::m_uavAddress),
                           MakeIpv4AddressChecker())
          .AddAttribute ("LocalAddress", "The UDP multicast address of this uav", Ipv4AddressValue(Ipv4Address((uint32_t) 0)),
                           MakeIpv4AddressAccessor(&UAV::m_local),
                           MakeIpv4AddressChecker())

          .AddAttribute("PacketInterval", "", TimeValue(Seconds(1)),
                           MakeTimeAccessor(&UAV::m_packetInterval),
                           MakeTimeChecker())
          .AddAttribute("CalculateInterval", "", TimeValue(Seconds(0.1)),
                           MakeTimeAccessor(&UAV::m_calculateInterval),
                           MakeTimeChecker())
          .AddAttribute ("UavCount", "The number of UAV's in the simulation. Used for finding ip addresses. Always >= 2 because of the central node + 1 client node", UintegerValue (2),
                         MakeUintegerAccessor (&UAV::m_uavCount),
                         MakeUintegerChecker<uint32_t> ())
          .AddAttribute ("UavType", "What type this uav is", UintegerValue (2),
                         MakeUintegerAccessor (&UAV::m_uavType),
                         MakeUintegerChecker<UAVDataType_> ())

          ;

  return tid;
}

UAV::UAV ()
{
  NS_LOG_FUNCTION (this);
}

UAV::~UAV ()
{
  NS_LOG_FUNCTION (this);
  m_socket = 0;
  NS_LOG_INFO("UAV: " << m_uavAddress << "received"); 
  for (const auto& entry : m_packetRecvCount) {
    NS_LOG_INFO("  " << entry.first << " - " << entry.second);
  }
  NS_LOG_INFO("UAV: " << m_uavAddress << "sent"); 
  for (const auto& entry : m_packetSendCount) {
    NS_LOG_INFO("  " << entry.first << " - " << entry.second);
  }
  NS_LOG_INFO(""); 
}

void
UAV::DoDispose (void)
{
  NS_LOG_FUNCTION (this);
  Application::DoDispose ();
}

void
UAV::StartApplication (void)
{
  NS_LOG_FUNCTION (this);

  if (m_socket == 0)
    {
      TypeId tid = TypeId::LookupByName ("ns3::UdpSocketFactory");
      m_socket = Socket::CreateSocket (GetNode (), tid);
      InetSocketAddress local = InetSocketAddress (m_uavAddress, m_port);
      if (m_socket->Bind (local) == -1)
        {
          NS_FATAL_ERROR ("Failed to bind socket");
        }
      if (addressUtils::IsMulticast (m_local))
        {
          Ptr<UdpSocket> udpSocket = DynamicCast<UdpSocket> (m_socket);
          if (udpSocket)
            {
              // equivalent to setsockopt (MCAST_JOIN_GROUP)
              udpSocket->MulticastJoinGroup (0, m_local);
            }
          else
            {
              NS_FATAL_ERROR ("Error: Failed to join multicast group");
            }
        }
    }

  m_socket->SetRecvCallback (MakeCallback (&UAV::HandleRead, this));
  m_socket->SetAllowBroadcast(true);

  m_sendEvent = Simulator::Schedule (Seconds(0.0), &UAV::Send, this);
  m_calculateEvent = Simulator::Schedule (Seconds(0.0), &UAV::Calculate, this);

  if (m_uavType == UAVDataType::VIRTUAL_FORCES_CENTRAL_POSITION) {
    SetColor(m_uavAddress, { 0.3, 0.7, 1.0 });
  }

  uint32_t lowAddress = m_uavAddress.Get() & 0xFF;
  if (ShouldDoCyberAttack() && lowAddress == 2) {
    //Have the number 2 node be the cyber attack because .1 is the central node
    Simulator::Schedule (Seconds(15.0), &UAV::Cyberattack, this);
  }
}

void UAV::Cyberattack() {
  NS_LOG_INFO("CYBERATTACK");
  m_uavType = UAVDataType::VIRTUAL_FORCES_CENTRAL_POSITION;
  SetColor(m_uavAddress, Vector(1.0, 0.2, 0.2));
}

void
UAV::StopApplication ()
{
  NS_LOG_FUNCTION (this);

  if (m_socket != 0)
    {
      m_socket->Close ();
      m_socket->SetRecvCallback (MakeNullCallback<void, Ptr<Socket>> ());
    }
}

void
UAV::HandleRead (Ptr<Socket> socket)
{

  Ptr<Packet> packet;
  Address from;
  Address localAddress;
  while ((packet = socket->RecvFrom (from)))
  {
    socket->GetSockName (localAddress);
    m_rxTrace (packet);
    m_rxTraceWithAddresses (packet, from, localAddress);
    Ptr<WaypointMobilityModel> mobility = GetNode()->GetObject<WaypointMobilityModel>(MobilityModel::GetTypeId());
    if (InetSocketAddress::IsMatchingType (from))
    {
      if (packet->GetSize() != sizeof(UAVData)) {
        //Drop packets that are not the correct size
        continue;
      }
      auto ipv4Addr = InetSocketAddress::ConvertFrom (from).GetIpv4 ();
      if (ipv4Addr == m_uavAddress) {
        continue;
      }
      m_packetRecvCount[ipv4Addr]++;
      
      UAVData data;
      packet->CopyData(reinterpret_cast<uint8_t*>(&data), sizeof(UAVData));
      
      auto& entry = m_swarmData[ipv4Addr];
      entry.data = data;
    }
    packet->RemoveAllPacketTags ();
    packet->RemoveAllByteTags ();

  }
}


void operator+=(Vector& a, const Vector& b) {
  a.x += b.x;
  a.y += b.y;
  a.z += b.z;
}

void operator-=(Vector& a, const Vector& b) {
  a.x -= b.x;
  a.y -= b.y;
  a.z -= b.z;
}

Vector operator*(const Vector& a, const Vector& b) {
  return { a.x * b.x, a.y * b.y, a.z * b.z };
}

Vector operator*(const Vector& a, double b) {
  return { a.x * b, a.y * b, a.z * b };
}

Vector operator/(const Vector& a, double b) {
  return { a.x / b, a.y / b, a.z / b };
}

Vector operator/(double a, const Vector& b) {
  return { a / b.x, a / b.y, a / b.z };
}

Vector operator-(const Vector& a) {
  return { -a.x, -a.y, -a.z };
}




void
UAV::Send (void)
{
  NS_ASSERT (m_sendEvent.IsExpired ());
  auto mobilityModel = this->GetNode()->GetObject<ns3::WaypointMobilityModel>();
  NS_ASSERT(mobilityModel->IsInitialized());

  UAVData payload;
  payload.position = mobilityModel->GetPosition();
  payload.type = m_uavType;

  Address localAddress;
  m_socket->GetSockName (localAddress);

  for (uint32_t i = 0; i < m_uavCount; i++) {
    Ipv4Address currentPeer(m_rootAddress.Get() + i);

    if (Ipv4Address(currentPeer) == localAddress) {
      //Don't send packets to ourselves
      continue;
    }
    auto addr = InetSocketAddress(currentPeer, m_port);
    m_socket->SendTo(reinterpret_cast<uint8_t*>(&payload), sizeof(payload), 0, addr);
    m_packetSendCount[currentPeer]++;
    m_sent++;

  }

  m_sendEvent = Simulator::Schedule (m_packetInterval, &UAV::Send, this);
}



//Math functions for linear interpolation and moving between ranges
template<typename T>
T lerp(T a, T b, T f) {
    //Convert the 0-1 range into a value in the right range.
    return a + (b - a) * f;
}


template<typename T>
T normalize(T a, T b, T value) {
    return (value - a) / (b - a);
}


template<typename T>
T map(T value, T leftMin, T leftMax, T rightMin, T rightMax) {
    // Figure out how 'wide' each range is
    T f = normalize(leftMin, leftMax, value);

    return lerp(rightMin, rightMax, f);
}



void UAV::Calculate() {
  auto mobilityModel = this->GetNode()->GetObject<ns3::WaypointMobilityModel>();

  Vector myPosition = mobilityModel->GetPosition();

  //NS_LOG_INFO("Me at " << myPosition);
  Vector attraction = { 0, 0, 0};
  Vector repulsion = { 0, 0, 0};
  for (auto& pair : m_swarmData) {
    auto& data = pair.second;

    //Unit vector points from us to the other node
    auto toOther = data.data.position - myPosition;
    double length = toOther.GetLength();
    toOther = toOther / length;

    //NS_LOG_INFO("  other at " << data.data.position << " other at " << toOther);
    if (m_uavType == UAVDataType::VIRTUAL_FORCES_POSITION && data.data.type == UAVDataType::VIRTUAL_FORCES_CENTRAL_POSITION) {

      //NS_LOG_INFO("  attracting to center" << toOther);
      //This could be simplified to attraction += data.data.position - myPosition;
      //But I leave it like this to clearly show the magnitude of the force and the direction seperately
      float force = length;
      attraction += toOther * force;

      //attraction += toOther;
    }
    if (m_uavType == UAVDataType::VIRTUAL_FORCES_POSITION && data.data.type == UAVDataType::VIRTUAL_FORCES_POSITION) {
      //Force is inversely proportional to length
      float force = 1.0 / length;
      //And points away from the other node
      repulsion += -toOther * force;
      //NS_LOG_INFO("  repulsing from other force: " << force << " in dir: " << toOther);
    }
  }

  //Apply phisics and integrate
  double dt = m_calculateInterval.GetSeconds();
  double mass = 1;
  //a=F/m
  Vector acceleration = (attraction * s_Parameters.a + repulsion * s_Parameters.r) / mass;
  m_velocity += acceleration * dt;

  auto now = Simulator::Now();
  auto later = now + m_calculateInterval;
  mobilityModel->AddWaypoint(Waypoint(later, myPosition + m_velocity * dt));

  //Slight velocity dampening if high enough
  double velocity = m_velocity.GetLength();
  const double minDampen = 0.2;
  const double maxDampen = 1.0;
  //Dampen at most 50% of overall velocity per second
  double maxDampenValue = 0.5 * dt;

  double dampening;
  if (velocity > maxDampen) {
    dampening = maxDampenValue;
  } else if (velocity > minDampen) {
    dampening = map(velocity, minDampen, maxDampen, 0.0, maxDampenValue);
  } else {
    //No dampening for velocities [0.0..minDampen] so nodes can get moving
    dampening = 0.0;
  }

  m_velocity -= m_velocity * dampening;
  
  m_calculateEvent = Simulator::Schedule (m_calculateInterval, &UAV::Calculate, this);

}

// ========== Helper stuff ==========


UAVHelper::UAVHelper (Ipv4Address serverAddress, uint16_t port, UAVDataType_ type, Time packetInterval, Time calculateInterval, uint32_t uavCount)
{
  m_factory.SetTypeId (UAV::GetTypeId ());
  SetAttribute("ServerAddress", Ipv4AddressValue(serverAddress));
  SetAttribute ("Port", UintegerValue (port));
  SetAttribute("PacketInterval", TimeValue(packetInterval));
  SetAttribute("CalculateInterval", TimeValue(calculateInterval));
  SetAttribute("UavCount", UintegerValue(uavCount));
  SetAttribute("UavType", UintegerValue(type));
}

void
UAVHelper::SetAttribute (std::string name, const AttributeValue &value)
{
  m_factory.Set (name, value);
}

ApplicationContainer
UAVHelper::Install (Ptr<Node> node) const
{
  return ApplicationContainer (InstallPriv (node));
}

ApplicationContainer
UAVHelper::Install (std::string nodeName) const
{
  Ptr<Node> node = Names::Find<Node> (nodeName);
  return ApplicationContainer (InstallPriv (node));
}

ApplicationContainer
UAVHelper::Install (NodeContainer c) const
{
  ApplicationContainer apps;
  for (NodeContainer::Iterator i = c.Begin (); i != c.End (); ++i)
    {
      apps.Add (InstallPriv (*i));
    }

  return apps;
}

Ptr<Application>
UAVHelper::InstallPriv (Ptr<Node> node) const
{
  Ptr<Application> app = m_factory.Create<UAV> ();
  node->AddApplication (app);

  return app;
}
