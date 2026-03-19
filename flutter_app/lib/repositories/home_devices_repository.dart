import '../models/device.dart';
import '../services/device_discovery_service.dart';
import '../services/device_storage_service.dart';

class HomeDevicesRepository {
  HomeDevicesRepository({DeviceStorageService? storage})
    : _storage = storage ?? DeviceStorageService();

  final DeviceStorageService _storage;

  Future<List<Device>> getStoredDevices() => _storage.getAll();

  Future<void> saveDevice(Device device) => _storage.save(device);

  Future<void> deleteDevice(String deviceId) => _storage.delete(deviceId);

  Future<Map<String, dynamic>> bind(String host, String phoneId) {
    return DeviceDiscoveryService.bind(host, phoneId);
  }

  Future<Map<String, dynamic>> getPairStatus(String host) {
    return DeviceDiscoveryService.getPairStatus(host);
  }
}
